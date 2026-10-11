struct S {
private:
  int hidden() const { return 7; }
public:
  int f(S& s) const {
    return [](S& x) -> int { return x.hidden(); }(s);
  }
};

int main() {
  S s;
  return s.f(s) - 7;
}
