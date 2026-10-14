// expect reject
// A completed friend's member signature must not grant access to the next function.
class Reader;
class Vault {
  typedef int Key;
  friend class Reader;
};
class Reader {
public:
  Vault::Key read();
};
Vault::Key Reader::read() { return 7; }
Vault::Key unrelated() { return 1; }
