# Topics & Wildcards

Topics are the addressing mechanism in MQTT. Publishers send messages to topics, and subscribers receive messages from topics they've subscribed to.

## Topic Names

A **topic name** is the destination for a published message.

### Rules

- UTF-8 encoded string
- At least 1 character
- Maximum 65,535 bytes
- Case-sensitive (`Sensor/Temp` ≠ `sensor/temp`)
- Can contain spaces (but not recommended)
- Cannot contain wildcards (`+` or `#`)
- Cannot contain null character

### Best Practices

```
✅ Good topics:
   home/kitchen/temperature
   sensors/device-01/status
   usa/california/sf/weather

❌ Avoid:
   /leading/slash           (creates empty first level)
   trailing/slash/          (creates empty last level)
   double//slash            (creates empty middle level)
   Spaces In Topic          (harder to work with)
```

### Topic Levels

Topics are hierarchical, separated by `/`:

```
home / kitchen / temperature
 │       │           │
 │       │           └── Level 3
 │       └────────────── Level 2
 └────────────────────── Level 1
```

## Topic Filters

A **topic filter** is used when subscribing. It can include wildcards to match multiple topics.

### Single-Level Wildcard (`+`)

Matches exactly **one** topic level.

```bash
# Subscribe
mqtt-consumer "home/+/temperature"
```

| Published Topic | Matches? |
|-----------------|----------|
| `home/kitchen/temperature` | ✅ Yes |
| `home/bedroom/temperature` | ✅ Yes |
| `home/temperature` | ❌ No (missing level) |
| `home/floor1/kitchen/temperature` | ❌ No (extra level) |

**More examples:**

| Filter | Matches |
|--------|---------|
| `+/tennis/#` | `sport/tennis/player1` |
| `sport/+/player1` | `sport/tennis/player1`, `sport/golf/player1` |
| `+/+` | `a/b`, `foo/bar` (exactly 2 levels) |

### Multi-Level Wildcard (`#`)

Matches **zero or more** topic levels. Must be the last character.

```bash
# Subscribe to everything under home/
mqtt-consumer "home/#"
```

| Published Topic | Matches? |
|-----------------|----------|
| `home` | ✅ Yes |
| `home/kitchen` | ✅ Yes |
| `home/kitchen/temperature` | ✅ Yes |
| `home/floor1/kitchen/oven/status` | ✅ Yes |
| `office/kitchen` | ❌ No |

**More examples:**

| Filter | Matches |
|--------|---------|
| `#` | Everything (all topics) |
| `sport/#` | `sport`, `sport/tennis`, `sport/tennis/player1` |
| `sport/tennis/#` | `sport/tennis`, `sport/tennis/player1/ranking` |

### Combining Wildcards

You can use both wildcards in one filter:

```bash
mqtt-consumer "+/sensors/+/temperature/#"
```

Matches:
- `home/sensors/kitchen/temperature`
- `office/sensors/room1/temperature/celsius`

## System Topics

Topics starting with `$` are reserved for broker internals:

```
$SYS/broker/version
$SYS/broker/uptime
$SYS/broker/clients/connected
```

- Clients **cannot** publish to `$` topics
- Wildcard `#` does **not** match `$` topics
- Must explicitly subscribe: `$SYS/#`

## Topic Design Patterns

### Hierarchical (Location-based)

```
{country}/{state}/{city}/{building}/{floor}/{room}/{sensor}
usa/california/sf/office1/floor2/room201/temperature
```

### Entity-based

```
{entity_type}/{entity_id}/{attribute}
device/sensor-001/temperature
user/alice/presence
```

### Event-based

```
{domain}/{event_type}/{entity}
orders/created/order-123
payments/processed/tx-456
```

### Command & Response

```
# Commands
device/{device_id}/command
device/{device_id}/command/reboot

# Responses
device/{device_id}/response
device/{device_id}/status
```

## Implementation Details

This broker uses a **trie data structure** for O(k) topic matching, where k is the number of topic levels. This enables efficient matching even with millions of subscriptions.

```
Root
├── home
│   ├── kitchen
│   │   └── temperature [subscriber: client-1]
│   └── + (wildcard)
│       └── humidity [subscriber: client-2]
└── sensors
    └── # (multi-wildcard) [subscriber: client-3]
```
