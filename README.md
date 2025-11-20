# eventually

A macOS menu bar application that displays upcoming calendar events.

## Overview

`eventually` is a lightweight status bar app that shows your next calendar event and provides quick access to your schedule.

## Features

- Displays next event time in the menu bar
- Shows upcoming events in a dropdown menu
- Dismiss events temporarily to see what's next
- Open current/upcoming events video calls

## Usage

Run the app:

```bash
eventually
```

On first launch, you'll need to grant calendar access in System Settings > Privacy & Security > Calendars.

The program also provides the means to setup a launchtl service for starting on login:

```bash
eventually service install/uninstall/start/stop/restart
```

## Permissions

Requires calendar access to read events from your default calendar. The app only reads data; it never modifies your calendar.

