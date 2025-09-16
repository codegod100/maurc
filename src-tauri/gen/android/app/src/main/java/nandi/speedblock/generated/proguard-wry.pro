# THIS FILE IS AUTO-GENERATED. DO NOT MODIFY!!

# Copyright 2020-2023 Tauri Programme within The Commons Conservancy
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT

-keep class nandi.speedblock.* {
  native <methods>;
}

-keep class nandi.speedblock.WryActivity {
  public <init>(...);

  void setWebView(nandi.speedblock.RustWebView);
  java.lang.Class getAppClass(...);
  java.lang.String getVersion();
}

-keep class nandi.speedblock.Ipc {
  public <init>(...);

  @android.webkit.JavascriptInterface public <methods>;
}

-keep class nandi.speedblock.RustWebView {
  public <init>(...);

  void loadUrlMainThread(...);
  void loadHTMLMainThread(...);
  void evalScript(...);
}

-keep class nandi.speedblock.RustWebChromeClient,nandi.speedblock.RustWebViewClient {
  public <init>(...);
}
