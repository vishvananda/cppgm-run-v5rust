struct CovariantPrefix {
  virtual ~CovariantPrefix();
};

struct CovariantBase {
  virtual ~CovariantBase();
  virtual CovariantBase * self();
};

struct CovariantDerived : CovariantPrefix, CovariantBase {
  CovariantDerived();
  CovariantDerived * self();
  int value;
};

CovariantBase * make_covariant_base();
