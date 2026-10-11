struct External {
  External();
  External(int v) : value(v) {}
  int value;
};

External::External() : value(9) {}
