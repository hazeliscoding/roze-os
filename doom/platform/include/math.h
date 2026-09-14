/* mini math.h. doom is fixed point, the trig table generators are
   ifdef zero in the source, only fabs is live (mouse acceleration).
   declarations exist so includes compile, implementations appear the
   day the linker asks for them. */

#ifndef ROZE_MATH_H
#define ROZE_MATH_H

double fabs(double x);
double sin(double x);
double cos(double x);
double tan(double x);
double atan(double x);
double sqrt(double x);
double pow(double base, double exp);

#endif
