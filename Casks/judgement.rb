cask "judgement" do
  version "0.2.0"
  sha256 "a7f64404749cb8a7016ca539b8d244e0d068d97a56a93ceb20a0cacef9be09a1"

  url "https://github.com/jordanpapaleo/judgement/releases/download/v#{version}/Judgement_#{version}_aarch64.dmg"
  name "Judgement"
  desc "Menu-bar store of engineering guidance for coding agents"
  homepage "https://github.com/jordanpapaleo/judgement"

  depends_on macos: :big_sur
  app "Judgement.app"
  binary "#{appdir}/Judgement.app/Contents/MacOS/judgement-cli", target: "judgement"
end
