// N3485 15.2p2, 12.2p3: when a constructor's member initializer, or a
// temporary it makes, throws, the members already constructed are destroyed in
// reverse order and the full-expression's temporaries once; a countdown throws
// at each step in turn
