// 物理のパネルとビューポートの表示が共有する表示の状態 (Undo の対象ではない)。
// 物理の設定そのものは EditorDocument.physics (バックエンドの Document の写し) にある。

export class PhysicsView {
  /** 物理のパネルを開いているか (開いている間、ビューポートに粒子と衝突体を重ねて衝突体を動かせる)。 */
  active = $state(false);
  /** パネルで選んでいるリグの ID。 */
  rig = $state<string | null>(null);
  /** パネルで選んでいる衝突体の ID。 */
  collider = $state<string | null>(null);
}
