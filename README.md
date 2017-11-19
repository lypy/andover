# Andover PTP plug
Lypy CNI plugin for addressless PTP links

## Overview
The andover plugin creates a point-to-point link between a container and the host by using a veth device.
One end of the veth pair is placed inside a container and the other end resides on the host.
This plugin provides only a ipv6 link-local addressing and does not use IPAM.
The traffic of the container interface will be routed through the interface of the host.

## Example network configuration

```
{
	"name": "mynet",
	"type": "andover",
	"andover": "andover",
}
```

## Network configuration reference

* `name` (string, required): the name of the network
* `type` (string, required): "andover"
* `andover` (string, required): Host side interface name
