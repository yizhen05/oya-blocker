#include <Arduino.h>
#include <WiFi.h>
#include <HTTPClient.h>
#include <Arduino_JSON.h>

// ----------------------------------------------
// ★★ Wi-Fi設定 ★★
const char* ssid = WIFI_SSID;
const char* password = WIFI_PASSWORD;

// ★★ Botが動いているPCのIPアドレスとポート ★★
const char* serverUrl = SERVER_URL;

// ★★ 出力ピンの設定 ★★
// XIAO ESP32C3の D0 ピンなどを接続先にしてください
// (リレーやLEDをつなぐピン)
const int RELAY_PIN = D0; 
// ----------------------------------------------

String lastStatus = ""; 

void setup() {
  // M5.begin() は削除し、シリアル通信を開始
  Serial.begin(115200);
  
  // ピン設定
  pinMode(RELAY_PIN, OUTPUT);
  digitalWrite(RELAY_PIN, LOW);

  Serial.println("Starting...");
  Serial.print("Connecting to WiFi: ");
  Serial.println(ssid);

  WiFi.begin(ssid, password);
  while (WiFi.status() != WL_CONNECTED) {
    delay(500);
    Serial.print(".");
  }
  Serial.println("\nWiFi Connected!");
  Serial.print("IP Address: ");
  Serial.println(WiFi.localIP());
}

void loop() {
  if (WiFi.status() == WL_CONNECTED) {
    HTTPClient http;
    http.begin(serverUrl);
    http.setConnectTimeout(2000);
    int httpCode = http.GET();

    if (httpCode == HTTP_CODE_OK) {
      String payload = http.getString();
      
      JSONVar myObject = JSON.parse(payload);
      if (JSON.typeof(myObject) == "object") {
        String currentStatus = (const char*) myObject["status"];

        // ステータス変化時のみログ出力・ピン制御
        if (currentStatus != lastStatus) {
          lastStatus = currentStatus;
          
          Serial.print("Status changed: ");
          Serial.println(currentStatus);

          if (currentStatus == "on") {
            // ONAIR: ピンをHIGHに
            digitalWrite(RELAY_PIN, HIGH);
          } else {
            // OFFLINE: ピンをLOWに
            digitalWrite(RELAY_PIN, LOW);
          }
        }
      }
    } else {
      // サーバー接続失敗
      if (lastStatus != "error") {
        Serial.printf("HTTP Error: %d\n", httpCode);
        lastStatus = "error";
      }
    }
    http.end();
  } else {
    // WiFi切断
    if (lastStatus != "wifi_error") {
      Serial.println("WiFi Disconnected");
      lastStatus = "wifi_error";
      // 必要ならここで再接続処理を入れる
      WiFi.reconnect(); 
    }
  }

  delay(3000); 
}