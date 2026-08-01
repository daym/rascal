unit u;
interface
type
  tsymtable = object
    procedure load;
  end;
  tunitsymtable = object(tsymtable)
    procedure loadrefs;
  end;
  punitsymtable = ^tunitsymtable;
  tmodule = record
    globalsymtable : pointer;
  end;
  pmodule = ^tmodule;
var current_module : pmodule;
procedure run;
implementation
procedure tsymtable.load;
begin
end;
procedure tunitsymtable.loadrefs;
begin
end;
procedure run;
begin
  punitsymtable(current_module^.globalsymtable)^.loadrefs;
end;
end.
