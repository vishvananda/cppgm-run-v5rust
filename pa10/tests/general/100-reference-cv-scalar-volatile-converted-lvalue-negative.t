// C++11 reference initialization: N3485 8.5.3.
int main(){int x=3;const volatile long& r=x;return r;}
