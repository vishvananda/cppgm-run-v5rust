// An ambiguous first phase cannot fall back to the scalar constructor.
// expect reject
namespace std { template<class T> class initializer_list; }
struct Pick {
  Pick(std::initializer_list<int>);
  Pick(std::initializer_list<long>);
  Pick(long, int);
};
int main() { Pick value{1L, 2}; }
