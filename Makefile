prefix = /usr

all: wp360-codesys-bridge

install: all
	install -d $(DESTDIR)/var/lib/CodeMeter/CmAct_ewf
	install -D wp360-codesys-bridge $(DESTDIR)$(prefix)/bin/wp360-codesys-bridge
	install -D wp360-codesys-stopswitch $(DESTDIR)$(prefix)/libexec/wp360-codesys-bridge/wp360-codesys-stopswitch
	install -D 19-stopswitch.conf $(DESTDIR)$(prefix)/lib/systemd/system/codesyscontrol.service.d/19-stopswitch.conf
	install -D CODESYSControl_WP360.cfg $(DESTDIR)/etc/codesyscontrol/CODESYSControl_WP360.cfg

clean:
	-rm -r target

.PHONY: all install clean
