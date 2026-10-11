class holder {
  typedef int value_type;
  static value_type value;
public:
  static int get() { return value; }
};
holder::value_type holder::value = 7;
int main() { return holder::get() == 7 ? 0 : 1; }
