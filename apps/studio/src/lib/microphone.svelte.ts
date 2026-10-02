// マイクの口パク (T0803、docs/studio.md#マイクの口パク)。操作ウィンドウの WebView でマイクの音を取り、口を開く量 (0〜1) を
// 指揮役のリップシンクの入力に送る。音そのものはどこにも送らない。
//
// 処理は `step` を外から呼んで進める (スタジオはバックエンドの刻みで呼ぶ。ウィンドウを最小化しても止まらないように)。
import { t } from "./i18n";
import type { Device } from "./inputs";
import { follow, type LipSyncTuning, loudness, rms, targetLevel } from "./lipSync";

export type MicrophoneStatus = "idle" | "loading" | "running" | "error";

/** これより小さな変化は送らない (IPC を減らす)。 */
const SEND_EPSILON = 0.002;
/** 1 回に進める時間の上限 (秒)。 */
const MAX_DT = 0.1;

/** getUserMedia の失敗を、利用者に分かる言葉にする (元のメッセージも付ける)。 */
function microphoneError(e: unknown): string {
  const name = e instanceof DOMException ? e.name : "";
  const reason =
    name === "NotAllowedError" || name === "SecurityError"
      ? t("microphone.errors.notAllowed")
      : name === "NotFoundError" || name === "OverconstrainedError"
        ? t("microphone.errors.notFound")
        : name === "NotReadableError"
          ? t("microphone.errors.busy")
          : t("microphone.errors.failed");
  return `${reason}: ${e instanceof Error ? e.message : String(e)}`;
}

export class Microphone {
  status = $state<MicrophoneStatus>("idle");
  error = $state<string | null>(null);
  devices = $state<Device[]>([]);
  /** 選んでいる機器 (空は既定の機器)。 */
  deviceId = $state("");
  /** 今の音の大きさ (0〜1。メーター)。 */
  loudness = $state(0);
  /** 送った口を開く量 (0〜1)。 */
  level = $state(0);

  readonly #send: (level: number | null) => void;
  readonly #tuning: () => LipSyncTuning;
  #stream: MediaStream | null = null;
  #context: AudioContext | null = null;
  #analyser: AnalyserNode | null = null;
  #buffer: Float32Array<ArrayBuffer> | null = null;
  #last: number | null = null;
  #sent: number | null = null;

  /** `send` は口を開く量 (`null` で止める)、`tuning` は今の感度と閾値を返す。 */
  constructor(send: (level: number | null) => void, tuning: () => LipSyncTuning) {
    this.#send = send;
    this.#tuning = tuning;
  }

  get running(): boolean {
    return this.status === "running";
  }

  /** 機器の一覧を更新する。名前は一度マイクを許可した後でないと空のことがある。 */
  async refreshDevices(): Promise<void> {
    if (!navigator.mediaDevices?.enumerateDevices) return;
    const devices = await navigator.mediaDevices.enumerateDevices();
    this.devices = devices
      .filter((d) => d.kind === "audioinput")
      .map((d, i) => ({
        id: d.deviceId,
        label: d.label || t("microphone.numbered", { index: i + 1 }),
      }));
  }

  /** 選んでいる機器の名前 (一覧に無ければ空)。 */
  get deviceLabel(): string {
    return this.devices.find((d) => d.id === this.deviceId)?.label ?? "";
  }

  /** 覚えた機器を選ぶ (ID が無ければ名前で探す。見つからなければ既定の機器)。 */
  choose(device: Device | null): void {
    if (device === null) {
      this.deviceId = "";
      return;
    }
    const found =
      this.devices.find((d) => d.id === device.id) ??
      this.devices.find((d) => d.label === device.label);
    this.deviceId = found?.id ?? "";
  }

  async start(): Promise<void> {
    this.stop();
    this.error = null;
    this.status = "loading";
    try {
      if (!navigator.mediaDevices?.getUserMedia) throw new Error(t("microphone.errors.noDevices"));
      const stream = await navigator.mediaDevices
        .getUserMedia({
          audio: {
            ...(this.deviceId ? { deviceId: { exact: this.deviceId } } : {}),
            // 声の大きさをそのまま使う (自動の音量調整などは口の開きを均してしまう)。
            echoCancellation: false,
            noiseSuppression: false,
            autoGainControl: false,
          },
          video: false,
        })
        .catch((e: unknown) => {
          throw new Error(microphoneError(e));
        });
      this.#stream = stream;
      const context = new AudioContext();
      const analyser = context.createAnalyser();
      analyser.fftSize = 1024;
      context.createMediaStreamSource(stream).connect(analyser);
      // 作った直後に止まっている (自動再生の制限) ことがあるので動かす。
      if (context.state === "suspended") await context.resume();
      this.#context = context;
      this.#analyser = analyser;
      this.#buffer = new Float32Array(analyser.fftSize);
      await this.refreshDevices();
      const id = stream.getAudioTracks()[0]?.getSettings().deviceId;
      if (id) this.deviceId = id;
    } catch (e) {
      this.#release();
      this.status = "error";
      this.error = e instanceof Error ? e.message : String(e);
      return;
    }
    this.#last = null;
    this.#sent = null;
    this.status = "running";
  }

  stop(): void {
    const wasRunning = this.status === "running";
    this.#release();
    this.status = "idle";
    this.loudness = 0;
    this.level = 0;
    if (wasRunning) this.#send(null);
    this.#sent = null;
  }

  /** 1 回分進める: 音の大きさを測り、口を開く量を追って、変わっていれば送る。 */
  step(now: number): void {
    const analyser = this.#analyser;
    const buffer = this.#buffer;
    if (this.status !== "running" || analyser === null || buffer === null) return;
    const dt = this.#last === null ? 0 : Math.min((now - this.#last) / 1000, MAX_DT);
    this.#last = now;
    analyser.getFloatTimeDomainData(buffer);
    const value = loudness(rms(buffer));
    this.loudness = value;
    const level = follow(this.level, targetLevel(value, this.#tuning()), dt);
    this.level = level;
    const sent = this.#sent;
    if (sent === null || Math.abs(level - sent) > SEND_EPSILON || (level === 0 && sent !== 0)) {
      this.#sent = level;
      this.#send(level);
    }
  }

  #release(): void {
    for (const track of this.#stream?.getTracks() ?? []) track.stop();
    this.#stream = null;
    void this.#context?.close().catch(() => undefined);
    this.#context = null;
    this.#analyser = null;
    this.#buffer = null;
  }
}
