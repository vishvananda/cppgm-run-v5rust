struct RefWrap {
  int& second;
};

RefWrap make_wrap(int& x) {
  return RefWrap{x};
}

int f() {
  int x = 0;
  RefWrap sb = make_wrap(x);
  return &sb.second == &x ? 0 : 1;
}

int main() { return f(); }
