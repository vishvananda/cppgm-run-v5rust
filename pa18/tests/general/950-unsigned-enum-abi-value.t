enum E:unsigned long long{huge=18446744073709551615ull};
template<E V>int value(){return 3;}int main(){return value<huge>()-3;}
