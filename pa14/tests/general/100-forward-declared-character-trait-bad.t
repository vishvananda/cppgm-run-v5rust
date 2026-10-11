namespace std { template<class T> struct char_traits; }
struct Character {};
typedef std::char_traits<Character>::int_type Integer;
int main() { return 0; }
