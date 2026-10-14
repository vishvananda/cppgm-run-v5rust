// expect reject
// An elaborated friend stops in its nearest namespace instead of naming an outer class.
class Reader;
namespace model {
class Secret {
  int value;
  friend class Reader;
};
}
class Reader {
public:
  int read(model::Secret & s) { return s.value; }
};
int main() { return 0; }
