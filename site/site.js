// The page in the reader's language, the picture of the desk in it too,
// and the clock of its menu bar.
"use strict";

const words = {
  "zh-Hans": {
    title: "Platinum — GPUI 的经典桌面控件",
    menuFile: "文件", menuEdit: "编辑", menuView: "显示", menuSpecial: "特别", menuHelp: "帮助", desk: "桌面",
    tagline: "GPUI 的经典桌面控件",
    about: "窗口、菜单、按钮、列表与对话框，九十年代末桌面的灰色立体风格，用于 <a href=\"https://github.com/longbridge/gpui-fast\">GPUI</a>。以 Rust 编写，逐像素绘制，支持五种语言。",
    language: "语言", browserLanguage: "浏览器的语言",
    install: "安装", source: "源码", docs: "文档", needs: "需要", license: "许可",
    docsText: "<a href=\"https://github.com/firede/gpui-platinum/blob/main/README.zh.md\">README</a>、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/api.md\">API 指南</a>（英文）、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/spec.md\">设计规范</a>（英文）",
    licenseText: "MIT；字体来自 <a href=\"https://github.com/firede/platinum-fonts\">Platinum Fonts</a>，OFL 1.1",
    sceneAlt: "示例桌面，开着几个窗口：磁盘、便笺、计算器和拼图。",
    caption: "示例桌面，由 GPUI 绘制。它是原生应用，不在浏览器中运行：<code>cargo run --example desktop</code>。",
    apiGuide: "API 指南",
  },
  "zh-Hant": {
    title: "Platinum — GPUI 的經典桌面控制項",
    menuFile: "檔案", menuEdit: "編輯", menuView: "顯示", menuSpecial: "特別", menuHelp: "輔助說明", desk: "桌面",
    tagline: "GPUI 的經典桌面控制項",
    about: "視窗、選單、按鈕、列表與對話框，九〇年代末桌面的灰色立體風格，用於 <a href=\"https://github.com/longbridge/gpui-fast\">GPUI</a>。以 Rust 撰寫，逐像素繪製，支援五種語言。",
    language: "語言", browserLanguage: "瀏覽器的語言",
    install: "安裝", source: "原始碼", docs: "文件", needs: "需要", license: "授權",
    docsText: "<a href=\"https://github.com/firede/gpui-platinum/blob/main/README.zh.md\">README</a>（簡體中文）、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/api.md\">API 指南</a>（英文）、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/spec.md\">設計規範</a>（英文）",
    licenseText: "MIT；字型來自 <a href=\"https://github.com/firede/platinum-fonts\">Platinum Fonts</a>，OFL 1.1",
    sceneAlt: "範例桌面，開著幾個視窗：磁碟、便箋、計算機和拼圖。",
    caption: "範例桌面，由 GPUI 繪製。它是原生應用程式，不在瀏覽器中執行：<code>cargo run --example desktop</code>。",
    apiGuide: "API 指南",
  },
  ja: {
    title: "Platinum — GPUI のクラシックなデスクトップコントロール",
    menuFile: "ファイル", menuEdit: "編集", menuView: "表示", menuSpecial: "特別", menuHelp: "ヘルプ", desk: "デスク",
    tagline: "GPUI のクラシックなデスクトップコントロール",
    about: "ウインドウ、メニュー、ボタン、リスト、ダイアログ。90 年代末のデスクトップのグレーで立体的なスタイルを、<a href=\"https://github.com/longbridge/gpui-fast\">GPUI</a> に。Rust で書かれ、1 ピクセルずつ描かれ、5 つの言語に対応。",
    language: "言語", browserLanguage: "ブラウザの言語",
    install: "インストール", source: "ソース", docs: "ドキュメント", needs: "必要なもの", license: "ライセンス",
    docsText: "<a href=\"https://github.com/firede/gpui-platinum/blob/main/README.md\">README</a>、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/api.md\">API ガイド</a>、<a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/spec.md\">デザイン仕様</a>（英語）",
    licenseText: "MIT。フォントは <a href=\"https://github.com/firede/platinum-fonts\">Platinum Fonts</a>（OFL 1.1）",
    sceneAlt: "いくつかのウインドウが開いたサンプルのデスクトップ：ディスク、メモ、計算機、スライドパズル。",
    caption: "GPUI が描いたサンプルのデスクトップ。ブラウザではなく、ネイティブで動きます：<code>cargo run --example desktop</code>",
    apiGuide: "API ガイド",
  },
  ko: {
    title: "Platinum — GPUI를 위한 클래식 데스크톱 컨트롤",
    menuFile: "파일", menuEdit: "편집", menuView: "보기", menuSpecial: "특수", menuHelp: "도움말", desk: "데스크",
    tagline: "GPUI를 위한 클래식 데스크톱 컨트롤",
    about: "윈도우, 메뉴, 버튼, 목록, 대화상자를 90년대 말 데스크톱의 회색 입체 스타일로, <a href=\"https://github.com/longbridge/gpui-fast\">GPUI</a>에. Rust로 작성되고 픽셀 단위로 그려지며 다섯 가지 언어를 지원합니다.",
    language: "언어", browserLanguage: "브라우저 언어",
    install: "설치", source: "소스", docs: "문서", needs: "필요 사항", license: "라이선스",
    docsText: "<a href=\"https://github.com/firede/gpui-platinum/blob/main/README.md\">README</a>, <a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/api.md\">API 가이드</a>, <a href=\"https://github.com/firede/gpui-platinum/blob/main/docs/spec.md\">디자인 명세</a>(영문)",
    licenseText: "MIT. 글꼴은 <a href=\"https://github.com/firede/platinum-fonts\">Platinum Fonts</a>(OFL 1.1)",
    sceneAlt: "창 몇 개가 열린 예제 데스크톱: 디스크, 메모, 계산기, 슬라이딩 퍼즐.",
    caption: "GPUI가 그린 예제 데스크톱. 브라우저가 아닌 네이티브로 실행됩니다: <code>cargo run --example desktop</code>",
    apiGuide: "API 가이드",
  },
};
// The words of the page in English, as it is written, for going back to
// them.
const scene = document.getElementById("scene");
const english = { title: document.title, sceneAlt: scene.alt };
for (const el of document.querySelectorAll("[data-t]")) english[el.dataset.t] = el.innerHTML;

// browserLanguage returns the first of the reader's languages the page
// speaks, or English.
function browserLanguage() {
  for (const tag of navigator.languages || [navigator.language || "en"]) {
    const t = tag.toLowerCase();
    if (t.startsWith("zh")) {
      return /hant|-tw|-hk|-mo/.test(t) ? "zh-Hant" : "zh-Hans";
    }
    if (t.startsWith("ja")) return "ja";
    if (t.startsWith("ko")) return "ko";
    if (t.startsWith("en")) return "en";
  }
  return "en";
}

// The language the reader chose, kept for the next visit, or "" for the
// browser's.
const chosenKey = "platinum-desk-language";
const picker = document.getElementById("language");
let chosen = "";
try { chosen = localStorage.getItem(chosenKey) || ""; } catch {}
if (!(chosen in words) && chosen !== "en") chosen = "";
let lang = chosen || browserLanguage();
const say = (key) => (words[lang] || english)[key] ?? english[key];

const clock = document.getElementById("clock");
let time;

// speak puts the page, and the picture of the desk, in lang.
function speak() {
  document.documentElement.lang = lang;
  document.title = say("title");
  for (const el of document.querySelectorAll("[data-t]")) el.innerHTML = say(el.dataset.t);
  scene.src = "scene-" + lang + ".png";
  scene.alt = say("sceneAlt");
  time = new Intl.DateTimeFormat(lang, { hour: "numeric", minute: "2-digit" });
  clock.textContent = time.format(new Date());
}

picker.value = chosen;
picker.addEventListener("change", () => {
  chosen = picker.value;
  try { chosen ? localStorage.setItem(chosenKey, chosen) : localStorage.removeItem(chosenKey); } catch {}
  lang = chosen || browserLanguage();
  speak();
});

// The menu bar's clock, as the page's language writes the time.
function tick() {
  clock.textContent = time.format(new Date());
  setTimeout(tick, 60000 - (Date.now() % 60000));
}

speak();
tick();
