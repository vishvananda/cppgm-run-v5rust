namespace std {template<class E>class initializer_list {const E* first;unsigned long count;initializer_list(const E*p,unsigned long n):first(p),count(n){} public:initializer_list():first(0),count(0){} unsigned long size()const{return count;} const E*begin()const{return first;} };}
int main(){const volatile auto& xs={1,2};return 0;}
