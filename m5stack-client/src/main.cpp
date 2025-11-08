#include <M5Stack.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <Arduino_JSON.h> // JSONパーサー

// ----------------------------------------------
// ★★ Wi-Fi設定 ★★
// (platformio.ini / secret.ini から注入される)
const char* ssid = WIFI_SSID;
const char* password = WIFI_PASSWORD;

// ★★ Botが動いているPCのIPアドレスとポート ★★
const char* serverUrl = SERVER_URL;
// ----------------------------------------------

String lastStatus = ""; // 前回のステータスを記憶

void setup() {
  M5.begin();

  // --- ★★★ ここから追加 ★★★ ---
  // GPIO 2 を出力 (OUTPUT) に設定
  pinMode(2, OUTPUT);
  // GPIO 2 の出力を LOW (0V) に設定
  digitalWrite(2, LOW);

  M5.Lcd.setTextSize(6); // 文字サイズを大きく
  M5.Lcd.fillScreen(BLACK);
  M5.Lcd.println("Connecting...");

  WiFi.begin(ssid, password);
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    M5.Lcd.print(".");
  }
  M5.Lcd.println("\nWiFi OK");
  delay(1000);
}

void loop() {
  if (WiFi.status() == WL_CONNECTED) {
    HTTPClient http;
    http.begin(serverUrl);
    http.setConnectTimeout(2000); // タイムアウトを短めに
    int httpCode = http.GET();

    if (httpCode == HTTP_CODE_OK) {
      String payload = http.getString();
      
      JSONVar myObject = JSON.parse(payload);
      if (JSON.typeof(myObject) == "object") {
        String currentStatus = (const char*) myObject["status"]; // (const char*) へのキャスト

        // ステータスに変化があった時だけ画面を更新
        if (currentStatus != lastStatus) {
          lastStatus = currentStatus;
          M5.Lcd.clear();
          M5.Lcd.setCursor(0, 100); // 画面中央あたりに
          
          if (currentStatus == "on") {
            M5.Lcd.setTextColor(RED, BLACK); // 赤文字
            M5.Lcd.println(" ONAIR ");
            // GPIO 2 の出力を HIGH (3.3V) に設定
            digitalWrite(2, HIGH);
          } else {
            M5.Lcd.setTextColor(GREEN, BLACK); // 緑文字
            M5.Lcd.println("OFFLINE");
            // GPIO 2 の出力を LOW (0V) に設定
            digitalWrite(2, LOW);
          }
        }
      }
    } else {
      // サーバーに接続失敗（Botが落ちてるなど）
      if (lastStatus != "error") {
        M5.Lcd.clear();
        M5.Lcd.setTextColor(YELLOW, BLACK);
        M5.Lcd.setTextSize(3);
        M5.Lcd.println("Connect Err");
        M5.Lcd.setTextSize(6); // サイズを元に戻す
        lastStatus = "error";
      }
    }
    http.end();
  } else {
    // WiFiが切れた
    if (lastStatus != "wifi_error") {
      M5.Lcd.clear();
      M5.Lcd.setTextColor(BLUE, BLACK);
      M5.Lcd.setTextSize(3);
      M5.Lcd.println("WiFi Error");
      M5.Lcd.setTextSize(6);
      lastStatus = "wifi_error";
    }
  }

  delay(3000); // 3秒ごとにステータスを確認
}