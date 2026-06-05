# Golden Audio Fixtures

Place 16 kHz mono PCM WAV files here for integration tests. Each file should be under 1 MB.

| File | Spoken content | Expected transcription contains |
|------|----------------|--------------------------------|
| `short_greeting.wav` | 你好 | 你好 |
| `number_amount.wav` | 一共一百二十三块五 | 123 |
| `date_sentence.wav` | 今天是二零二六年六月五日 | 2026 |
| `mixed_en_cn.wav` | 打开 Chrome 浏览器 | Chrome |
| `long_paragraph.wav` | ~30 s continuous speech | (non-empty) |

## Recording tips

1. Use 16 kHz sample rate, mono, 16-bit PCM.
2. Record in a quiet room; avoid background music.
3. Run tests locally after downloading models:

```bash
./scripts/download-models.sh
cd src-tauri && cargo test --test asr_golden -- --ignored
```

Fixtures are not committed by default (large binary). Copy your recordings into this directory before running golden tests.
