import SwiftUI
import Translation

@available(macOS 15.0, *)
struct TranslationSetupView: View {
    @State private var descriptionConfiguration: TranslationSession.Configuration? = .init(
        source: Locale.Language(identifier: "en"),
        target: Locale.Language(identifier: "zh-Hans")
    )
    @State private var searchConfiguration: TranslationSession.Configuration?
    @State private var message = "正在准备 Apple 英译中简介语言能力…"
    @State private var finished = false
    @State private var succeeded = false

    var body: some View {
        VStack(alignment: .leading, spacing: 16) {
            Text("启用本机中英文搜索与简介翻译")
                .font(.title2.bold())
            Text("语言包由 macOS 管理。简介和中文搜索词都只在这台 Mac 上翻译。")
                .foregroundStyle(.secondary)
            HStack(spacing: 10) {
                if finished {
                    Image(systemName: succeeded ? "checkmark.circle.fill" : "xmark.circle.fill")
                        .foregroundStyle(succeeded ? .green : .red)
                } else {
                    ProgressView()
                }
                Text(message)
            }
            if finished {
                Button("完成") { NSApplication.shared.terminate(nil) }
                    .keyboardShortcut(.defaultAction)
            }
        }
        .padding(24)
        .frame(width: 470, height: 210)
        .translationTask(descriptionConfiguration) { session in
            do {
                try await session.prepareTranslation()
                message = "英译中已准备好，正在准备中文搜索…"
                searchConfiguration = .init(
                    source: Locale.Language(identifier: "zh-Hans"),
                    target: Locale.Language(identifier: "en")
                )
            } catch {
                message = "未能准备英译中语言能力：\(error.localizedDescription)"
                finished = true
                succeeded = false
            }
        }
        .translationTask(searchConfiguration) { session in
            do {
                try await session.prepareTranslation()
                message = "简介翻译和中文搜索都已准备好，可以返回 BrewDesk。"
                finished = true
                succeeded = true
            } catch {
                message = "未能准备中译英搜索能力：\(error.localizedDescription)"
                finished = true
                succeeded = false
            }
        }
    }
}

@main
struct BrewDeskTranslationSetupApp: App {
    var body: some Scene {
        WindowGroup {
            if #available(macOS 15.0, *) {
                TranslationSetupView()
            } else {
                Text("当前 macOS 版本不支持 Apple 本机翻译。")
                    .padding(24)
            }
        }
        .windowResizability(.contentSize)
    }
}
