"use strict";

const translations = {
  "zh-Hans": {
    title: "GPUI Platinum — Rust 经典桌面控件",
    description: "面向 Rust 和 GPUI 的像素绘制 Platinum 控件，提供原生控件图库、经典桌面示例、API 指南和设计规范。",
    skip: "跳到正文", navigation: "主导航", gallery: "图库", desktop: "桌面", documentation: "文档",
    aboutTitle: "关于 GPUI Platinum", tagline: "Rust 与 GPUI 的经典桌面控件",
    about: "窗口、菜单、按钮、列表与对话框，延续九十年代末桌面的灰色立体风格。以 Rust 编写，通过 GPUI 原生逐像素绘制。",
    language: "语言", automatic: "浏览器的语言", needs: "需要", license: "许可", fontLicense: "附有点阵字体许可", source: "源码", readme: "README 与源码指南",
    copy: "复制", copied: "命令已复制。", copyFailed: "请选中命令并复制。", nativeNote: "在本地运行示例。下方图库展示的是本 GPUI 实现。", getStarted: "开始使用", viewGallery: "查看图库",
    galleryTitle: "控件图库", galleryIntro: "原生图库按五页展示各类控件。", galleryPages: "图库页面", page1: "基础控件", page2: "输入与文字", page3: "集合", page4: "日期与表格", page5: "导航",
    galleryCaption1: "按钮、选择控件、滑杆、菜单与窗口框架。", galleryCaption2: "斜面按钮、工具栏、输入框、状态显示与富文本。", galleryCaption3: "列表、树、可排序表格、图标与布局。", galleryCaption4: "日历、日期时间、颜色、电子表格与标尺。", galleryCaption5: "文档标签、分栏、路径、字体选择与页面设置。", openGallery: "打开图库原尺寸图片",
    desktopTitle: "经典桌面", desktopIntro: "一个 GPUI 窗口里的一整套桌面：虚拟磁盘、可移动窗口、计算器、便笺、富文本、电子表格和控制面板。", openDesktop: "打开桌面原尺寸图片", desktopCaption: "原生桌面示例，简体中文界面。", desktopNote: "示例的文件保存在自己的内存磁盘中。控件库不替代操作系统桌面。",
    usingTitle: "使用控件库", usingIntro: "初始化一次，将有状态控件保留为 GPUI 实体，由应用状态构建轻量控件。", usingNote: "应用内的 GPUI 依赖应固定到同一修订版。0.x 阶段 API 仍可能变化。", englishReadme: "英文介绍", apiGuide: "API 指南", apiDescription: "状态、事件、焦点与本地化。", designSpec: "设计规范", specDescription: "灰阶、尺寸与控件交互。", examples: "示例源码", examplesDescription: "用这些控件构建的原生桌面。", referenceNote: "从当前检出的源码生成 Rust 参考文档。", footer: "GPUI 版本，源自", tribute: "仅在风格上致敬经典桌面，与 Apple Inc. 无关。",
  },
  "zh-Hant": {
    title: "GPUI Platinum — Rust 經典桌面控制項",
    description: "適用於 Rust 與 GPUI 的像素繪製 Platinum 控制項，提供原生控制項圖庫、經典桌面範例、API 指南與設計規範。",
    skip: "跳至內文", navigation: "主要導覽", gallery: "圖庫", desktop: "桌面", documentation: "文件",
    aboutTitle: "關於 GPUI Platinum", tagline: "Rust 與 GPUI 的經典桌面控制項",
    about: "視窗、選單、按鈕、列表與對話框，延續九〇年代末桌面的灰色立體風格。以 Rust 撰寫，透過 GPUI 原生逐像素繪製。",
    language: "語言", automatic: "瀏覽器的語言", needs: "需要", license: "授權", fontLicense: "附有點陣字型授權", source: "原始碼", readme: "README 與原始碼指南",
    copy: "複製", copied: "指令已複製。", copyFailed: "請選取指令並複製。", nativeNote: "在本機執行範例。下方圖庫展示的是本 GPUI 實作。", getStarted: "開始使用", viewGallery: "查看圖庫",
    galleryTitle: "控制項圖庫", galleryIntro: "原生圖庫以五頁展示各類控制項。", galleryPages: "圖庫頁面", page1: "基本控制項", page2: "輸入與文字", page3: "集合", page4: "日期與試算表", page5: "導覽",
    galleryCaption1: "按鈕、選擇控制項、滑桿、選單與視窗框架。", galleryCaption2: "斜面按鈕、工具列、輸入框、狀態顯示與富文字。", galleryCaption3: "列表、樹狀檢視、可排序表格、圖示與版面配置。", galleryCaption4: "月曆、日期時間、顏色、試算表與尺規。", galleryCaption5: "文件分頁、分欄、路徑、字型選擇與頁面設定。", openGallery: "開啟圖庫原尺寸圖片",
    desktopTitle: "經典桌面", desktopIntro: "一個 GPUI 視窗裡的一整套桌面：虛擬磁碟、可移動視窗、計算機、便箋、富文字、試算表與控制台。", openDesktop: "開啟桌面原尺寸圖片", desktopCaption: "原生桌面範例，畫面為英文介面。", desktopNote: "範例的檔案保存在自己的記憶體磁碟中。控制項庫不替代作業系統桌面。",
    usingTitle: "使用控制項庫", usingIntro: "初始化一次，將有狀態控制項保留為 GPUI 實體，由應用程式狀態建立輕量控制項。", usingNote: "應用程式內的 GPUI 相依套件應固定到同一修訂版。0.x 階段 API 仍可能變動。", englishReadme: "英文介紹", apiGuide: "API 指南", apiDescription: "狀態、事件、焦點與在地化。", designSpec: "設計規範", specDescription: "灰階、尺寸與控制項互動。", examples: "範例原始碼", examplesDescription: "用這些控制項建立的原生桌面。", referenceNote: "從目前簽出的原始碼產生 Rust 參考文件。", footer: "GPUI 版本，源自", tribute: "僅在風格上致敬經典桌面，與 Apple Inc. 無關。",
  },
  ja: {
    title: "GPUI Platinum — Rust のクラシックなデスクトップコントロール",
    description: "Rust と GPUI のためのピクセル描画 Platinum コントロール。ネイティブのギャラリー、デスクトップのサンプル、API ガイド、デザイン仕様を紹介します。",
    skip: "本文へ移動", navigation: "メインナビゲーション", gallery: "ギャラリー", desktop: "デスクトップ", documentation: "ドキュメント",
    aboutTitle: "GPUI Platinum について", tagline: "Rust と GPUI のクラシックなデスクトップコントロール",
    about: "ウインドウ、メニュー、ボタン、リスト、ダイアログ。90 年代末のグレーで立体的なデスクトップのスタイルを Rust で実装し、GPUI で 1 ピクセルずつ描画します。",
    language: "言語", automatic: "ブラウザの言語", needs: "必要なもの", license: "ライセンス", fontLicense: "ビットマップフォントのライセンスを同梱", source: "ソース", readme: "README とソースガイド",
    copy: "コピー", copied: "コマンドをコピーしました。", copyFailed: "コマンドを選択してコピーしてください。", nativeNote: "サンプルはローカルで実行できます。下の画像はこの GPUI 実装の画面です。", getStarted: "使い始める", viewGallery: "ギャラリーを見る",
    galleryTitle: "コントロールギャラリー", galleryIntro: "ネイティブのギャラリーで、各種コントロールを 5 ページにまとめています。", galleryPages: "ギャラリーのページ", page1: "基本", page2: "入力と文字", page3: "コレクション", page4: "日付とシート", page5: "ナビゲーション",
    galleryCaption1: "ボタン、選択コントロール、スライダー、メニュー、ウインドウ枠。", galleryCaption2: "ベベルボタン、ツールバー、入力欄、ステータス表示、リッチテキスト。", galleryCaption3: "リスト、ツリー、並べ替え可能な表、アイコン、レイアウト。", galleryCaption4: "カレンダー、日時、色、スプレッドシート、ルーラー。", galleryCaption5: "ドキュメントタブ、カラム、パス、フォント選択、ページ設定。", openGallery: "ギャラリーの画像を原寸で開く",
    desktopTitle: "クラシックデスクトップ", desktopIntro: "1 つの GPUI ウインドウに、仮想ディスク、移動できるウインドウ、計算機、メモ、リッチテキスト、スプレッドシート、コントロールパネルをまとめています。", openDesktop: "デスクトップの画像を原寸で開く", desktopCaption: "ネイティブのデスクトップサンプル。画像は英語のインターフェースです。", desktopNote: "サンプルのファイルは専用のメモリ内ディスクに保存されます。このライブラリは OS のデスクトップを置き換えません。",
    usingTitle: "ライブラリの使い方", usingIntro: "一度初期化し、状態を持つコントロールを GPUI エンティティとして保持します。軽量なコントロールはアプリの状態から構築します。", usingNote: "アプリ内の GPUI 依存関係は同じリビジョンに固定してください。0.x の API は変更されることがあります。", englishReadme: "英語の概要", apiGuide: "API ガイド", apiDescription: "状態、イベント、フォーカス、ローカライズ。", designSpec: "デザイン仕様", specDescription: "グレーの階調、寸法、コントロールの動作。", examples: "サンプルのソース", examplesDescription: "このコントロールで作られたネイティブのデスクトップ。", referenceNote: "チェックアウトしたソースから Rust のリファレンスを生成できます。", footer: "GPUI 版の原作：", tribute: "スタイルへのオマージュです。Apple Inc. とは関係ありません。",
  },
  ko: {
    title: "GPUI Platinum — Rust용 클래식 데스크톱 컨트롤",
    description: "Rust와 GPUI를 위한 픽셀 기반 Platinum 컨트롤. 네이티브 컨트롤 갤러리, 클래식 데스크톱 예제, API 가이드와 디자인 명세를 살펴보세요.",
    skip: "본문으로 이동", navigation: "주 메뉴", gallery: "갤러리", desktop: "데스크톱", documentation: "문서",
    aboutTitle: "GPUI Platinum 정보", tagline: "Rust와 GPUI용 클래식 데스크톱 컨트롤",
    about: "윈도우, 메뉴, 버튼, 목록과 대화상자에 90년대 말 데스크톱의 회색 입체 스타일을 담았습니다. Rust로 작성하고 GPUI로 픽셀 단위로 그리는 네이티브 컨트롤입니다.",
    language: "언어", automatic: "브라우저 언어", needs: "필요 사항", license: "라이선스", fontLicense: "비트맵 글꼴 라이선스 포함", source: "소스", readme: "README 및 소스 가이드",
    copy: "복사", copied: "명령을 복사했습니다.", copyFailed: "명령을 선택한 후 복사하세요.", nativeNote: "예제는 로컬에서 실행하세요. 아래 갤러리는 이 GPUI 구현의 화면입니다.", getStarted: "시작하기", viewGallery: "갤러리 보기",
    galleryTitle: "컨트롤 갤러리", galleryIntro: "네이티브 갤러리의 다섯 페이지에서 다양한 컨트롤을 보여 줍니다.", galleryPages: "갤러리 페이지", page1: "기본", page2: "입력과 텍스트", page3: "컬렉션", page4: "날짜와 시트", page5: "탐색",
    galleryCaption1: "버튼, 선택 컨트롤, 슬라이더, 메뉴와 윈도우 프레임.", galleryCaption2: "입체 버튼, 도구 모음, 입력란, 상태 표시와 서식 있는 텍스트.", galleryCaption3: "목록, 트리, 정렬 가능한 표, 아이콘과 레이아웃.", galleryCaption4: "달력, 날짜와 시간, 색상, 스프레드시트와 눈금자.", galleryCaption5: "문서 탭, 열 보기, 경로, 글꼴 선택과 페이지 설정.", openGallery: "갤러리 이미지를 원본 크기로 열기",
    desktopTitle: "클래식 데스크톱", desktopIntro: "하나의 GPUI 윈도우에 가상 디스크, 이동 가능한 창, 계산기, 메모, 서식 있는 텍스트, 스프레드시트와 제어판을 담았습니다.", openDesktop: "데스크톱 이미지를 원본 크기로 열기", desktopCaption: "네이티브 데스크톱 예제. 이미지는 영어 인터페이스입니다.", desktopNote: "예제의 파일은 자체 메모리 디스크에 저장됩니다. 이 라이브러리는 운영체제 데스크톱을 대체하지 않습니다.",
    usingTitle: "라이브러리 사용", usingIntro: "한 번 초기화하고 상태가 있는 컨트롤을 GPUI 엔티티로 유지하세요. 가벼운 컨트롤은 앱 상태에서 구성합니다.", usingNote: "앱의 GPUI 의존성은 같은 리비전으로 고정하세요. 0.x API는 바뀔 수 있습니다.", englishReadme: "영문 소개", apiGuide: "API 가이드", apiDescription: "상태, 이벤트, 포커스와 현지화.", designSpec: "디자인 명세", specDescription: "회색 계조, 크기와 컨트롤 동작.", examples: "예제 소스", examplesDescription: "이 컨트롤로 만든 네이티브 데스크톱.", referenceNote: "체크아웃한 소스에서 Rust 레퍼런스를 생성하세요.", footer: "GPUI 버전의 원작:", tribute: "스타일에 대한 헌정이며 Apple Inc.와 관련이 없습니다.",
  },
};

const english = {
  title: document.title,
  description: document.querySelector('meta[name="description"]').content,
  copied: "Command copied.",
  copyFailed: "Select the command and copy it.",
  galleryCaption2: "Bevel buttons, toolbar, fields, status displays and rich text.",
  galleryCaption3: "Lists, trees, sortable tables, icons and layout.",
  galleryCaption4: "Calendars, dates, time, colors, spreadsheets and rulers.",
  galleryCaption5: "Document tabs, columns, paths, font selection and page setup.",
};
for (const element of document.querySelectorAll("[data-t]")) {
  english[element.dataset.t] = element.textContent;
}
for (const element of document.querySelectorAll("[data-label]")) {
  english[element.dataset.label] = element.getAttribute("aria-label");
}

const languagePicker = document.getElementById("language");
const tabs = [...document.querySelectorAll("[data-page]")];
const languageKey = "gpui-platinum-language";
const supportedLanguages = ["en", ...Object.keys(translations)];
const query = new URLSearchParams(location.search);
let chosenLanguage = query.get("lang") || "";
if (!supportedLanguages.includes(chosenLanguage)) {
  try { chosenLanguage = localStorage.getItem(languageKey) || ""; } catch { chosenLanguage = ""; }
}
if (!supportedLanguages.includes(chosenLanguage)) chosenLanguage = "";
let language = chosenLanguage || browserLanguage();
let page = Math.min(5, Math.max(1, Number.parseInt(query.get("page"), 10) || 1));
let clockFormat;

function browserLanguage() {
  for (const value of navigator.languages || [navigator.language || "en"]) {
    const tag = value.toLowerCase();
    if (tag.startsWith("zh")) return /hant|-tw|-hk|-mo/.test(tag) ? "zh-Hant" : "zh-Hans";
    if (tag.startsWith("ja")) return "ja";
    if (tag.startsWith("ko")) return "ko";
    if (tag.startsWith("en")) return "en";
  }
  return "en";
}

function say(key) {
  return translations[language]?.[key] ?? english[key] ?? key;
}

function renderGallery() {
  const imagePath = `../docs/gallery${page === 1 ? "" : `-${page}`}@2x.png`;
  const caption = say(`galleryCaption${page}`);
  const image = document.getElementById("gallery-image");
  image.src = imagePath;
  image.alt = caption;
  document.getElementById("gallery-image-link").href = imagePath;
  document.getElementById("gallery-caption").textContent = caption;
  document.getElementById("gallery-panel").setAttribute("aria-labelledby", `page-tab-${page}`);
  for (const tab of tabs) {
    const selected = Number(tab.dataset.page) === page;
    tab.setAttribute("aria-selected", String(selected));
    tab.tabIndex = selected ? 0 : -1;
  }
}

function renderLanguage() {
  document.documentElement.lang = language;
  document.title = say("title");
  document.querySelector('meta[name="description"]').content = say("description");
  document.querySelector('meta[property="og:title"]').content = say("title");
  document.querySelector('meta[property="og:description"]').content = say("description");
  for (const element of document.querySelectorAll("[data-t]")) {
    element.textContent = say(element.dataset.t);
  }
  for (const element of document.querySelectorAll("[data-label]")) {
    element.setAttribute("aria-label", say(element.dataset.label));
  }
  const desktopPath = language === "zh-Hans" ? "../docs/desktop-zh-hans@2x.png" : "../docs/desktop@2x.png";
  document.getElementById("desktop-image").src = desktopPath;
  document.getElementById("desktop-image").alt = say("desktopIntro");
  document.getElementById("desktop-image-link").href = desktopPath;
  document.getElementById("copy-status").textContent = "";
  clockFormat = new Intl.DateTimeFormat(language, { hour: "numeric", minute: "2-digit" });
  renderClock();
  renderGallery();
}

function renderClock() {
  const now = new Date();
  const clock = document.getElementById("clock");
  clock.textContent = clockFormat.format(now);
  clock.dateTime = now.toISOString();
}

languagePicker.value = chosenLanguage;
languagePicker.addEventListener("change", () => {
  chosenLanguage = languagePicker.value;
  try {
    if (chosenLanguage) localStorage.setItem(languageKey, chosenLanguage);
    else localStorage.removeItem(languageKey);
  } catch { /* Language selection also works without browser storage. */ }
  language = chosenLanguage || browserLanguage();
  renderLanguage();
});

for (const tab of tabs) {
  tab.addEventListener("click", () => {
    page = Number(tab.dataset.page);
    renderGallery();
  });
  tab.addEventListener("keydown", (event) => {
    let target;
    if (event.key === "ArrowRight") target = page % tabs.length + 1;
    else if (event.key === "ArrowLeft") target = (page + tabs.length - 2) % tabs.length + 1;
    else if (event.key === "Home") target = 1;
    else if (event.key === "End") target = tabs.length;
    else return;
    event.preventDefault();
    page = target;
    renderGallery();
    tabs[page - 1].focus();
  });
}

document.getElementById("copy").addEventListener("click", async () => {
  const command = document.getElementById("run-command");
  let copied = false;
  try {
    await navigator.clipboard.writeText(command.textContent);
    copied = true;
  } catch {
    const selection = window.getSelection();
    const range = document.createRange();
    range.selectNodeContents(command);
    selection.removeAllRanges();
    selection.addRange(range);
    try { copied = document.execCommand("copy"); } catch { copied = false; }
    if (copied) selection.removeAllRanges();
  }
  document.getElementById("copy-status").textContent = say(copied ? "copied" : "copyFailed");
});

renderLanguage();
setInterval(renderClock, 60000);
