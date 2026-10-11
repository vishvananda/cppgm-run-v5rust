struct HiddenLeft {
  virtual ~HiddenLeft();
};

struct HiddenRight {
  int payload;

  HiddenRight();
  virtual ~HiddenRight();
};

HiddenLeft * make_hidden_left();
