struct RefWrap {
  int& second;
};

int f() {
  int x = 0;
  RefWrap sb = {x};
  return &sb.second == &x ? 0 : 1;
}

int main() { return f(); }
