// expect exit 0
// N3485 11p6/11.3: out-of-class member signatures keep the friend class context.
class Reader;
class Vault {
  typedef int Key;
  static const int seed = 7;
  friend class Reader;
};
class Reader {
public:
  Vault::Key read(Vault::Key);
  static Vault::Key seed;
  class Nested {
  public:
    Vault::Key read();
  };
};
Vault::Key Reader::read(Vault::Key n = Vault::seed) { return n; }
int main() {
  Reader reader;
  return reader.read() == 7 ? 0 : 1;
}
