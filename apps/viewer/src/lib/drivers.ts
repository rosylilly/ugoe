// パラメータを毎フレーム外から動かす仕組み (フェイストラッキング T0703、自動の目パチなど)。
//
// ドライバは `Player.drivers.add()` で登録する。フレームごとに、登録順にすべてのドライバの
// `frame(dt)` を呼び、返された値を設定してから評価・描画する。同じパラメータを複数のドライバが
// 返したら、後に登録したものが勝つ。ドライバが値を返しているパラメータは、スライダーで動かせない。

export type ParameterValues = Iterable<readonly [id: string, value: number]>;

export interface ParameterDriver {
  /** 表示用の名前。 */
  readonly name: string;
  /**
   * 毎フレーム呼ばれ、このフレームで設定する値 (ID と値) を返す。返さなかったパラメータには触れない。
   * `dt` は前のフレームからの秒数。
   */
  frame(dt: number): ParameterValues | null | undefined;
}

export class DriverSet {
  #drivers: ParameterDriver[] = [];

  /** ドライバを登録する。戻り値を呼ぶと登録を外す。 */
  add(driver: ParameterDriver): () => void {
    this.#drivers.push(driver);
    return () => {
      this.#drivers = this.#drivers.filter((d) => d !== driver);
    };
  }

  get size(): number {
    return this.#drivers.length;
  }

  /** すべてのドライバを 1 フレーム進め、設定する値をまとめる (後に登録したものが勝つ)。 */
  collect(dt: number): Map<string, number> {
    const out = new Map<string, number>();
    for (const driver of this.#drivers) {
      for (const [id, value] of driver.frame(dt) ?? []) {
        if (Number.isFinite(value)) out.set(id, value);
      }
    }
    return out;
  }
}
