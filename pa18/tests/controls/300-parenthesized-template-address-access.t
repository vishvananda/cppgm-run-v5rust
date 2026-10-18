// expect reject
// PA18: target-directed template address selection checks access at the use,
// retaining the parentheses outside the qualified address.
struct Base {
private:
  template<class T> static int choose(T);
};

int (*selected)(int) = ((&Base::choose));
