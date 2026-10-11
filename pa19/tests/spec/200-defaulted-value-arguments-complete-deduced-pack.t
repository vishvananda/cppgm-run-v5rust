// Completed integral defaults participate in the same pack deduction as explicit values.
template<int A=1,int B=A+1,int C=B+1,int D=C+1>struct box{};
template<int T,int...Ts>constexpr int count(const box<T,Ts...>&){return sizeof...(Ts);}
static_assert(count(box<>())==3,"all value defaults");
static_assert(count(box<7,8>())==3,"partial value defaults");
static_assert(count(box<7,8,9,10>())==3,"explicit values");
// Retain the value expansion in the function name as well as its concrete values.
int (*count_address)(const box<7>&)=&count<7,8,9,10>;
int main(){return count(box<7>())!=3 || count_address(box<7>())!=3;}
