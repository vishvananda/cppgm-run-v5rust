// expect reject
struct Source {
private:
  explicit operator int() { return 7; }
};
int main() { Source source; return source.operator int(); }
