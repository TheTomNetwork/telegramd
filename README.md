# Telegramd

## Get started

Clone the repo and run to compile the program

```bash
cargo build --release
```

To run the program simply execute the binary under `target/release/telegramd`

## Setup

Create a file `.env` and add a line such as the following

```.env
TELOXIDE_TOKEN=<your-telegram-bot-token>
```

## Routes

The web-server is listening on port `5005`

### `/send-message`

Either use a `GET` request and use URL-arguments or `PUSH` request with JSON-body.

```bash
curl 'localhost:5005/send-message?chatid=42&message=hi'
```

OR

```bash
curl 'localhost:5005/send-message' -d '{"chatid": "42", "message": "hi"}'
```

### `/send-file`

Use a PUT request to send a file such as the following:

```bash
curl -X PUT -F "file=@cool_pdf.pdf" 'http://localhost:5005/send-file?chatid=42'
```

It is also possible to send a message with the file:

```bash
curl -X PUT -F "file=@cool_pdf.pdf" 'http://localhost:5005/send-file?chatid=42&message=test'
```

You can also send several files

```bash
curl -X PUT -F "file=@cool_pdf.pdf" -F "file2=@other_pdf.pdf" 'http://localhost:5005/send-file?chatid=42&message=test'
```
