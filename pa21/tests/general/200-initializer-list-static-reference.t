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
const IL& get(){static const IL& held={1,2};return held;}
const IL& get_value(){static IL held={3,4};return held;}
int main(){
 const IL& held=get();
 if(alive!=2||held.size()!=2||held.begin()[0].value!=1||&held!=&get()||made!=2)return 1;
 const IL& value=get_value();
 return alive!=4||value.size()!=2||value.begin()[0].value!=3||
        value.begin()[1].value!=4||&value!=&get_value()||made!=4||destroyed!=0;
}
