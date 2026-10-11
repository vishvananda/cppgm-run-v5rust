struct X {
private:
  int m;
public:
  static X make() { X x = {2}; return x; }
  int get() const { return m; }
};

int main() { return X::make().get(); }
