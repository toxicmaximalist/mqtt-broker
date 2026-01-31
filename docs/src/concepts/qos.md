# Quality of Service (QoS)

QoS levels define the delivery guarantee between a client and the broker.

## Overview

| Level | Name | Guarantee | Use Case |
|-------|------|-----------|----------|
| 0 | At most once | Fire and forget | Telemetry, non-critical data |
| 1 | At least once | Guaranteed, may duplicate | Important events |
| 2 | Exactly once | Guaranteed, no duplicates | Financial transactions |

## QoS 0 — At Most Once

The simplest level. Message is sent once with no acknowledgment.

```
Publisher                Broker                 Subscriber
    │                       │                       │
    │  PUBLISH (QoS 0)      │                       │
    │ ────────────────────► │                       │
    │                       │  PUBLISH (QoS 0)      │
    │                       │ ────────────────────► │
    │                       │                       │
```

**Characteristics:**
- Fastest (no round trips)
- No retransmission
- Message may be lost
- No duplicate detection needed

**When to use:**
- Sensor readings at high frequency
- Data where occasional loss is acceptable
- When bandwidth/latency is critical

## QoS 1 — At Least Once

Message is guaranteed to arrive, but may arrive multiple times.

```
Publisher                Broker                 Subscriber
    │                       │                       │
    │  PUBLISH (QoS 1)      │                       │
    │  packet_id: 1         │                       │
    │ ────────────────────► │                       │
    │                       │  PUBLISH (QoS 1)      │
    │                       │  packet_id: 1         │
    │                       │ ────────────────────► │
    │                       │                       │
    │                       │  PUBACK               │
    │                       │  packet_id: 1         │
    │                       │ ◄──────────────────── │
    │  PUBACK               │                       │
    │  packet_id: 1         │                       │
    │ ◄──────────────────── │                       │
```

**Characteristics:**
- One round trip (PUBLISH → PUBACK)
- Message stored until acknowledged
- Retransmitted if no PUBACK received
- Duplicates possible on retransmit

**When to use:**
- Important events that must arrive
- When duplicates can be handled (idempotent operations)
- Status updates, alerts

## QoS 2 — Exactly Once

Most reliable level. Guarantees exactly one delivery.

```
Publisher                Broker                 Subscriber
    │                       │                       │
    │  PUBLISH (QoS 2)      │                       │
    │  packet_id: 1         │                       │
    │ ────────────────────► │                       │
    │                       │                       │
    │  PUBREC               │                       │
    │  packet_id: 1         │                       │
    │ ◄──────────────────── │                       │
    │                       │  PUBLISH (QoS 2)      │
    │                       │ ────────────────────► │
    │                       │  PUBREC               │
    │                       │ ◄──────────────────── │
    │  PUBREL               │                       │
    │  packet_id: 1         │                       │
    │ ────────────────────► │                       │
    │                       │  PUBREL               │
    │                       │ ────────────────────► │
    │                       │  PUBCOMP              │
    │                       │ ◄──────────────────── │
    │  PUBCOMP              │                       │
    │  packet_id: 1         │                       │
    │ ◄──────────────────── │                       │
```

**Characteristics:**
- Two round trips (4-way handshake)
- Highest latency
- No duplicates
- State must be tracked

**When to use:**
- Financial transactions
- Billing events
- Critical commands
- When duplicates are unacceptable

## QoS Downgrade

The actual QoS is the **minimum** of:
1. Publisher's QoS
2. Subscriber's maximum QoS

```
Publisher QoS    Subscriber Max QoS    Actual Delivery
     2                   0                    0
     2                   1                    1
     1                   2                    1
     0                   2                    0
```

Example:
```bash
# Publisher sends with QoS 2
mqtt-producer --qos 2 "topic" "message"

# Subscriber subscribed with QoS 1
mqtt-consumer --qos 1 "topic"

# Message delivered with QoS 1 (minimum)
```

## Packet Identifiers

QoS 1 and 2 use packet IDs to track message flows:

- 16-bit unsigned integer (1-65535)
- Unique per client per direction
- Reusable after flow completes
- 0 is reserved (not used)

## State Machines

### QoS 1 Publisher State

```
┌─────────────┐     PUBLISH     ┌─────────────┐
│   Idle      │ ──────────────► │  Awaiting   │
│             │                 │   PUBACK    │
└─────────────┘                 └──────┬──────┘
       ▲                               │
       │            PUBACK             │
       └───────────────────────────────┘
```

### QoS 2 Publisher State

```
┌─────────────┐     PUBLISH     ┌─────────────┐
│   Idle      │ ──────────────► │  Awaiting   │
│             │                 │   PUBREC    │
└─────────────┘                 └──────┬──────┘
       ▲                               │ PUBREC
       │                               ▼
       │                        ┌─────────────┐
       │          PUBCOMP       │  Awaiting   │
       └─────────────────────── │   PUBCOMP   │
                                └──────┬──────┘
                                       │ PUBREL
                                       ▼
```

## Performance Considerations

| Aspect | QoS 0 | QoS 1 | QoS 2 |
|--------|-------|-------|-------|
| Packets | 1 | 2 | 4 |
| Latency | Lowest | Medium | Highest |
| Bandwidth | Lowest | Medium | Highest |
| Memory | None | Moderate | Higher |
| Reliability | None | High | Highest |

## Best Practices

1. **Default to QoS 0** for high-frequency telemetry
2. **Use QoS 1** for important data where duplicates are okay
3. **Use QoS 2 sparingly** — only when exactly-once is required
4. **Design for idempotency** — prefer QoS 1 + idempotent handlers over QoS 2
5. **Consider trade-offs** — QoS 2 is 4× the packets of QoS 0
