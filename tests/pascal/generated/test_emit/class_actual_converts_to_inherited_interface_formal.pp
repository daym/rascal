unit u;
interface
{$interfaces corba}
type
  ireader = interface ['{11111111-1111-1111-1111-111111111111}']
    procedure next;
  end;
  tbase = class(tobject, ireader)
    procedure next;
  end;
  treader = class(tbase)
    procedure run;
  end;
procedure take(reader : ireader);
implementation
procedure tbase.next;
begin
end;
procedure take(reader : ireader);
begin
end;
procedure treader.run;
begin
  take(self);
end;
end.
