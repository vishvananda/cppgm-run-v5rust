namespace std {
template<class E> class initializer_list {
 const E* first; unsigned long count;
 initializer_list(const E* p,unsigned long n):first(p),count(n){}
 public:
 initializer_list():first(0),count(0){}
 unsigned long size()const{return count;}
 const E* begin()const{return first;}
 const E* end()const{return first+count;}
};
}
int alive,made,destroyed;
struct S { int value; S(int v)noexcept:value(v){++alive;++made;} ~S()noexcept{--alive;++destroyed;} };
using IL=std::initializer_list<S>;
int main(){const std::initializer_list<int>& held={1,2};return held.size()!=2||held.begin()[0]!=1||held.begin()[1]!=2;}
