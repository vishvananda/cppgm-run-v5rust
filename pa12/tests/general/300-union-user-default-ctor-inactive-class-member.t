struct NoDefault {
  NoDefault(NoDefault const &);
};

union Storage {
  NoDefault value;
  Storage() {}
  ~Storage() {}
};

int main() {
  Storage s;
  (void)s;
  return 0;
}
