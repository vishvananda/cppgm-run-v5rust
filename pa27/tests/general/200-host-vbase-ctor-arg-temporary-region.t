// host-object link: N3485 12.6.2p10, 15.2p2: F() : E("e", 5), M("y")
// constructs the virtual base V, then the other bases from temporaries with
// destructors; when a later base's construction throws, the virtual base
// already constructed is destroyed
