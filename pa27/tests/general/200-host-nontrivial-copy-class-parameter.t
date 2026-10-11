// Host-object ABI: a class with a non-trivial copy or move constructor, or with
// every copy and move constructor deleted, is passed by invisible reference in
// both directions, and the parameter is the object the constructor initialized.
