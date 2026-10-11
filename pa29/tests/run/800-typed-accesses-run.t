// N3485 3.10p10, 1.9p15: a store through an int glvalue does not change a
// pointer object, nor a store through a long glvalue a double: each later read
// gives the object's own value, and the stores take effect
