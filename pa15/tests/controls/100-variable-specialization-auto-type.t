// Explicit auto deduction belongs to the declaration; expect exit 0
template<class T> int value = 2;
template<> auto value<int> = 3L;
using First = decltype(value<int>);
using Again = decltype(value<int>);
static_assert(sizeof(First) == sizeof(long), "deduced declaration type");
static_assert(sizeof(Again) == sizeof(First), "canonical specialization type");
int main() { return value<int> - 3; }
