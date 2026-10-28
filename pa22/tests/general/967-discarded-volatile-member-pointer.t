struct S { volatile int value; };
void dot(S& s,volatile int S::*p){s.*p;}
void arrow(S* s,volatile int S::*p){s->*p;}
void cast(S& s,volatile int S::*p){(void)(s.*p);}
void wrapped(bool b,S& s,volatile int S::*p){b?s.*p:s.value;(s.*p,0);}
int main(){
  S value={7};volatile int S::*p=&S::value;
  dot(value,p);arrow(&value,p);cast(value,p);wrapped(true,value,p);
  return value.value-7;
}
