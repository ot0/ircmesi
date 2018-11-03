irc mesi

参加者募集、ログ、httpサーバ機能つき、IRCボット

使い方
robot.tomlを使用環境に合わせて編集

データベースの作成
sqlite3 mesi.db < migrations/2018-10-08-072059_create_post/up.sql

プログラムの実行
mesi

irc 上での操作
参加者募集を作成
mesi add <hoge>
    hoge:募集の名称

参加者募集の一覧表示
mesi shows

参加表明
mesi [id] + [name]
    id: 募集のid、省略した場合、直近に変更された募集
    name: 参加者の名前

参加辞退
mesi [id] - [name]
    id: 募集のid、省略した場合、直近に変更された募集
    name: 参加者の名前

募集の名称変更
mesi [id] title hoge
    id: 募集のid、省略した場合、直近に変更された募集
    hoge:募集の名称

webサーバの動作
-参加者募集の一覧の表示
-ダイスを振る他botに向けて2D6の出力
-直近のircメッセージ
-ログファイルへのリンク

トップページのデザインは,次のファイルを修正してください。
templates/index.hbs
修正後、mesiの再起動が必要です。

logで指定したディレクトリのファイルはトップページにリンクされます。
resourcesのディレクトリも公開されます。

ダイスを振る機能は連打による攻撃に対応するため、
10秒に1度しか動作しません。

コマンドラインオプション
mesi [option]

オプション
    -s setting.toml: 設定ファイルの指定。指定しない場合 robot.toml

robot.toml
[irc]
# サーバのアドレスとポート
server ="irc.org:6667" 
# サーバパスワード
password = ""
# ニックネーム。すでに使われていた場合、プログラムは終了します。
nick = "robot2"
# 参加するチャンネル
channel = "#test"

[log]
# ログを保存するディレクトリ
# また、webサーバで公開されます。
dir = "log"

[webs]
# webサーバのアドレス
host = "0.0.0.0:9619"
# ベーシック認証のユーザネーム
username = "mesi"
# ベーシック認証のパスワード
password = "mesi"
# set pkcs12 file, if https connected else ""
# ""の場合、webサーバはhttpとして動作
# サーバをhttps動作させる場合、pkcs12ファイルを指定。
# pkcs12のパスワードは空、変更する場合はプログラムを修正。
pem = ""

参加者募集のDB
sqlite3で、mesi.dbに保存します。
