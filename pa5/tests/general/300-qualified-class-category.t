struct A{template<class T> struct C;}; struct A::C{using V=int;}; void f(){A::C<int>::V *p;}
