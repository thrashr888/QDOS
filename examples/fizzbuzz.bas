10 REM FizzBuzz - Classic programming puzzle
20 PRINT "FizzBuzz 1 to 30:"
30 PRINT
40 FOR N = 1 TO 30
50   LET F = 0
60   IF N - INT(N/3)*3 = 0 THEN PRINT "Fizz";
70   IF N - INT(N/3)*3 = 0 THEN LET F = 1
80   IF N - INT(N/5)*5 = 0 THEN PRINT "Buzz";
90   IF N - INT(N/5)*5 = 0 THEN LET F = 1
100  IF F = 0 THEN PRINT N;
110  PRINT
120 NEXT N
130 END
