// expect reject
class Object {
public:
  Object() {}
private:
  ~Object() {}
};
int main() { static_cast<void>(Object()); }
