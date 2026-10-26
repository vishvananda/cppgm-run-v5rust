template<class T>auto raise(T value)->decltype(throw value){throw value;}int main(){try{raise(3);}catch(int x){return x-3;}return 1;}
