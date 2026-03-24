# USB Meteor Capture (L-Connect 3)

This guide captures and analyzes USB traffic for Lian Li Meteor effect using Wireshark CLI tools.

## Prerequisites

- Wireshark installed with CLI tools (`tshark`, `capinfos`) and USBPcap support.
- L-Connect 3 running.
- Target hardware connected (wireless dongle TX/RX, commonly `VID_0416&PID_8040` / `VID_0416&PID_8041`).

## 1) Guided Capture

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\capture-usb-meteor.ps1
```

The script writes files into `artifacts\usb-captures`:

- `<timestamp>-idle-baseline.pcapng`
- `<timestamp>-meteor-action.pcapng`
- `<timestamp>-action-timeline.txt`

During action capture it gives cues:

- T+10s: click **Apply** for Meteor
- T+40s: do nothing
- T+70s: click **Apply** again

## 2) Analyze Capture

Run:

```powershell
powershell -ExecutionPolicy Bypass -File .\scripts\analyze-usb-meteor.ps1 `
  -ActionCapture .\artifacts\usb-captures\<timestamp>-meteor-action.pcapng `
  -IdleCapture .\artifacts\usb-captures\<timestamp>-idle-baseline.pcapng
```

Output is written to `artifacts\usb-analysis`:

- `<timestamp>-meteor-report.md`
- `<timestamp>-meteor-target-rows.csv`

## Notes

- If `0416:8040/8041` is not found in the capture, analysis falls back to `345f:9132`.
- In the MVP UI, only these Meteor controls are user-adjustable: effect, color, speed.
- Brightness and smoothness are fixed internally for Meteor to keep dongle output consistent.
- Enumeration-only traces are expected to show mostly endpoint `0x00`/`0x80` and setup requests like `bRequest=6/9`.
- For stronger semantic mapping, run additional captures that change exactly one UI parameter per run (speed and color).
