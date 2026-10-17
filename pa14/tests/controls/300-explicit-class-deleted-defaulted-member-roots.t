// expect exit 0
// A class root does not synthesize a destructor deleted by inaccessible
// subobject destruction or an assignment deleted by subobject copy rules.
class private_destructor {
  ~private_destructor() {}
};
struct moving {
  moving(moving&&) = delete;
};
template<class T> struct destruction_box {
  T value;
  ~destruction_box() = default;
};
template<class T> struct assignment_box {
  T value;
  assignment_box& operator=(const assignment_box&) = default;
};
template struct destruction_box<private_destructor>;
template struct assignment_box<moving>;
int main() { return 0; }
