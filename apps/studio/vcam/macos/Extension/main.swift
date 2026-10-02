// Camera Extension の入口 (Camera.swift)。OS (CoreMediaIO の registerassistantservice) が起動し、プロバイダを出して待つ。

import CoreMediaIO
import Foundation

let provider = Provider(clientQueue: nil)
CMIOExtensionProvider.startService(provider: provider.provider)
CFRunLoopRun()
