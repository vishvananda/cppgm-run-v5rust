// Frozen types-mode special-member declaration responsibility regression.
template<class T>struct C{operator T(){T n;return n;}};
