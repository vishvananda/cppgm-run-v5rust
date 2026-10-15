// expect reject
class Object {
public:
  Object() {}
private:
  ~Object() {}
};
int consume(const Object&);
int main() { return consume(Object()); }
