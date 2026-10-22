// Host guards retry an initializer that unwinds, but remain released when
// an initializer temporary's destructor throws after construction completes.
