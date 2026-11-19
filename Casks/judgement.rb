cask "judgement" do
  version "0.2.0"
  sha256 "1f4afb3286e2913905aa12f3088c4fb88ae7f0bee60b2fb09471c75149860674"

  url "https://github.com/jordanpapaleo/judgement/releases/download/v#{version}/Judgement_#{version}_aarch64.dmg"
  name "Judgement"
  desc "Menu-bar store of engineering guidance for coding agents"
  homepage "https://github.com/jordanpapaleo/judgement"

  depends_on macos: :big_sur
  app "Judgement.app"
  binary "#{appdir}/Judgement.app/Contents/MacOS/judgement-cli", target: "judgement"
end
