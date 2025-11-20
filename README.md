# eventually

A macOS menu bar application that displays upcoming calendar events.

## Features

- Displays next event time in the menu bar
- Shows upcoming events in a dropdown menu
- Open current event's video call
- Open current event in calendar app
- Dismiss events 

## Usage

Run the app:

```bash
eventually
```

The program also provides the means to setup a launchtl service for launching on startup:

```bash
eventually service install/uninstall/start/stop/restart
```

## Permissions

On first launch, you'll need to grant calendar access in System Settings > Privacy & Security > Calendars.
Requires calendar access to read events from your default calendar. The app only reads data; it never modifies your calendar.

