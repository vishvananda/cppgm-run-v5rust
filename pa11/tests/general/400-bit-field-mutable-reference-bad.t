struct S { unsigned x:2; };
int consume(unsigned&);
int consume(const unsigned&);
int main(){S s;s.x=1;return consume(s.x);}
