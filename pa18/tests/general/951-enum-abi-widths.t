enum Small:unsigned char{small=255};enum Signed:signed char{negative=-1};enum Wide:unsigned __int128{wide=18446744073709551615ull};
template<Small V>int a(){return 1;}template<Signed V>int b(){return 2;}template<Wide V>int c(){return 3;}int main(){return a<small>()+b<negative>()+c<wide>()-6;}
