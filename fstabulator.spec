# Binary-only spec: scripts/install_to_rpm.sh pre-builds the release binary
# with cargo (embedding LOCALEDIR=/usr/share/locale) and passes
# cargo_version / fstab_srcdir / fstab_locales via --define.
Name:		fstabulator
Version:	%{cargo_version}
Release:	1%{?dist}
Summary:	GTK4 GUI for editing /etc/fstab
License:	GPL-3.0-or-later
URL:		https://github.com/LapisSea/fstabulator
# Building the project on the host additionally needs cargo, gtk4-devel,
# libadwaita-devel, glib2-devel and gettext.

Requires:	gtk4 >= 4.12
Requires:	libadwaita >= 1.9
Requires:	glib2
Requires:	polkit
Requires:	util-linux
Recommends:	shadow-utils
Suggests:	btrfs-progs

%description
FSTabulator is a GTK4/libadwaita front end for /etc/fstab. It lists the
block devices and filesystems on the system, edits mount entries, keeps
timestamped backups, and applies changes (mount, remount, unmount, swap)
through a polkit-authenticated root helper.

%install
install -d %{buildroot}%{_bindir}
install -m 0755 %{fstab_srcdir}/target/release/fstabulator %{buildroot}%{_bindir}/fstabulator

install -d %{buildroot}%{_datadir}/applications
install -m 0644 %{fstab_srcdir}/resources/org.lapissea.FSTabulator.desktop \
	%{buildroot}%{_datadir}/applications/org.lapissea.FSTabulator.desktop

install -d %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 %{fstab_srcdir}/resources/fstabulator_icon.svg \
	%{buildroot}%{_datadir}/icons/hicolor/scalable/apps/fstabulator.svg

install -d %{buildroot}%{_datadir}/icons/Adwaita-dark/scalable/apps
install -m 0644 %{fstab_srcdir}/resources/fstabulator_icon_dark.svg \
	%{buildroot}%{_datadir}/icons/Adwaita-dark/scalable/apps/fstabulator.svg
install -m 0644 %{fstab_srcdir}/resources/index.theme \
	%{buildroot}%{_datadir}/icons/Adwaita-dark/index.theme

install -d %{buildroot}%{_datadir}/polkit-1/actions
install -m 0644 %{fstab_srcdir}/resources/org.lapissea.FSTabulator.root-helper.policy \
	%{buildroot}%{_datadir}/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy

install -d %{buildroot}%{_datadir}/license/%{name}
install -m 0644 %{fstab_srcdir}/LICENSE %{buildroot}%{_datadir}/license/%{name}/LICENSE

if [ -d %{fstab_locales} ]; then
	for lang in %{fstab_locales}/*; do
		[ -d "$lang/LC_MESSAGES" ] || continue
		install -d %{buildroot}%{_datadir}/locale/$(basename "$lang")/LC_MESSAGES
		install -m 0644 "$lang/LC_MESSAGES/fstabulator.mo" \
			%{buildroot}%{_datadir}/locale/$(basename "$lang")/LC_MESSAGES/fstabulator.mo
	done
fi

%files
%license %{_datadir}/license/%{name}/LICENSE
%{_bindir}/fstabulator
%{_datadir}/applications/org.lapissea.FSTabulator.desktop
%{_datadir}/icons/hicolor/scalable/apps/fstabulator.svg
%{_datadir}/icons/Adwaita-dark/scalable/apps/fstabulator.svg
%{_datadir}/icons/Adwaita-dark/index.theme
%{_datadir}/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy
%{_datadir}/locale/*/LC_MESSAGES/fstabulator.mo

%changelog
* Sat Aug 29 2026 LapisSea - 0.1.0-1
- Initial package
