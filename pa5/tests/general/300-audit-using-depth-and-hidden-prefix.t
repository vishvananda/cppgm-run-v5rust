namespace Imported { typedef int word; }
namespace Prefix { typedef int word; }
struct ClassPrefix { typedef int word; };
void f()
{
  int word;
  {
    using namespace Imported;
    word * p;
    int Prefix;
    Prefix::word * q;
    int ClassPrefix;
    ClassPrefix::word * r;
    struct ClassPrefix * s;
    ClassPrefix::word * t;
  }
}
