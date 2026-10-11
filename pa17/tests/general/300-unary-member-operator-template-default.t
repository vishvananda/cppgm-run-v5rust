struct Box {
  int value;

  template<class Ref = int &, class Result = Ref>
  Result operator*() {
    return value;
  }
};

int main() {
  Box box = {3};
  int & ref = *box;
  ref = 9;
  return box.value == 9 ? 0 : 1;
}
