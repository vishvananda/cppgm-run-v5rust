namespace std {template<class E>class initializer_list {const E* first;unsigned long count;initializer_list(const E*p,unsigned long n):first(p),count(n){} public:initializer_list():first(0),count(0){} unsigned long size()const{return count;} const E*begin()const{return first;} };}
int main(){const auto& xs={1,2};return xs.size()!=2||xs.begin()[0]!=1||xs.begin()[1]!=2;}
