cask "judgement" do
  version "0.1.0"
  sha256 "0000000000000000000000000000000000000000000000000000000000000000"

  url "https://github.com/jordanpapaleo/judgement/releases/download/v#{version}/Judgement_#{version}_aarch64.dmg"
  name "Judgement"
  desc "Menu-bar store of engineering guidance for coding agents"
  homepage "https://github.com/jordanpapaleo/judgement"

  depends_on macos: :big_sur
  app "Judgement.app"
  binary "#{appdir}/Judgement.app/Contents/MacOS/judgement-cli", target: "judgement"
end
