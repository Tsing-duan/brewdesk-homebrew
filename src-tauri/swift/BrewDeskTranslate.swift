import Foundation
import Translation

struct TranslationInput: Decodable {
    let texts: [String]
}

struct TranslationOutput: Encodable {
    let available: Bool
    let installed: Bool
    let searchInstalled: Bool
    let status: String
    let message: String
    let translations: [String]?
}

@main
struct BrewDeskTranslate {
    static func emit(_ output: TranslationOutput) {
        let encoder = JSONEncoder()
        guard let data = try? encoder.encode(output), let json = String(data: data, encoding: .utf8) else {
            print(#"{"available":false,"installed":false,"searchInstalled":false,"status":"error","message":"无法编码翻译结果","translations":null}"#)
            return
        }
        print(json)
    }

    static func main() async {
        guard #available(macOS 26.0, *) else {
            emit(TranslationOutput(
                available: false,
                installed: false,
                searchInstalled: false,
                status: "unavailable",
                message: "当前 macOS 版本不支持 BrewDesk 的本机翻译功能",
                translations: nil
            ))
            return
        }

        let english = Locale.Language(identifier: "en")
        let simplifiedChinese = Locale.Language(identifier: "zh-Hans")
        let availability = LanguageAvailability()
        let descriptionStatus = await availability.status(from: english, to: simplifiedChinese)
        let searchStatus = await availability.status(from: simplifiedChinese, to: english)
        let descriptionInstalled = descriptionStatus == .installed
        let searchInstalled = searchStatus == .installed

        if CommandLine.arguments.contains("--status") {
            let available = descriptionStatus != .unsupported || searchStatus != .unsupported
            let fullyInstalled = descriptionInstalled && searchInstalled
            emit(TranslationOutput(
                available: available,
                installed: descriptionInstalled,
                searchInstalled: searchInstalled,
                status: fullyInstalled ? "installed" : available ? "download-required" : "unsupported",
                message: fullyInstalled
                    ? "Apple 英译中简介与中译英搜索语言能力均已安装"
                    : available
                        ? "需要准备 Apple 英译中简介与中译英搜索语言能力"
                        : "Apple 本机翻译不支持所需语言组合",
                translations: nil
            ))
            return
        }

        let translatingQuery = CommandLine.arguments.contains("--translate-query")
        let source = translatingQuery ? simplifiedChinese : english
        let target = translatingQuery ? english : simplifiedChinese
        let selectedInstalled = translatingQuery ? searchInstalled : descriptionInstalled

        guard selectedInstalled else {
            emit(TranslationOutput(
                available: translatingQuery ? searchStatus != .unsupported : descriptionStatus != .unsupported,
                installed: descriptionInstalled,
                searchInstalled: searchInstalled,
                status: "download-required",
                message: translatingQuery
                    ? "需要先在 BrewDesk 设置中准备 Apple 本机中译英搜索语言能力"
                    : "需要先在 BrewDesk 设置中准备 Apple 本机英译中简介语言能力",
                translations: nil
            ))
            return
        }

        do {
            let inputData = FileHandle.standardInput.readDataToEndOfFile()
            let input = try JSONDecoder().decode(TranslationInput.self, from: inputData)
            let session = TranslationSession(installedSource: source, target: target)
            var translations: [String] = []
            translations.reserveCapacity(input.texts.count)
            for text in input.texts.prefix(20) {
                let response = try await session.translate(text)
                translations.append(response.targetText)
            }
            emit(TranslationOutput(
                available: true,
                installed: descriptionInstalled,
                searchInstalled: searchInstalled,
                status: "installed",
                message: translatingQuery ? "搜索词由 Apple Translation 在本机翻译" : "简介由 Apple Translation 在本机翻译",
                translations: translations
            ))
        } catch {
            emit(TranslationOutput(
                available: true,
                installed: descriptionInstalled,
                searchInstalled: searchInstalled,
                status: "error",
                message: "本机翻译失败：\(error.localizedDescription)",
                translations: nil
            ))
        }
    }
}
