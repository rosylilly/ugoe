// `<ugoe-avatar>` を登録する入口 (`import "@ugoe/runtime/element"`、または `<script type="module">` で読む)。
// 登録しないで使う (別の名前にするなど) ときは、`@ugoe/runtime` の `defineUgoeAvatar` を使う。

import { defineUgoeAvatar } from "./avatar.js";

export {
  defineUgoeAvatar,
  UgoeAvatarElement,
  type UgoeAvatarElementEventMap,
  type UgoeAvatarErrorDetail,
  type UgoeAvatarLoadDetail,
} from "./avatar.js";

defineUgoeAvatar();
