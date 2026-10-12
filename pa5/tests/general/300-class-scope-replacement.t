struct C{using Old=int;}; struct C{using New=int;}; void f(){C::Old *p; C::New *q;}
