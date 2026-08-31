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
cat > %{buildroot}%{_datadir}/applications/org.lapissea.FSTabulator.desktop <<'EOF'
[Desktop Entry]
Type=Application
Name=FSTabulator
Comment=Edit /etc/fstab
Exec=fstabulator
Icon=fstabulator
Terminal=false
Categories=System;Utility;
EOF

install -d %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 0644 %{fstab_srcdir}/resources/fstabulator_icon.svg \
	%{buildroot}%{_datadir}/icons/hicolor/scalable/apps/fstabulator.svg

install -d %{buildroot}%{_datadir}/icons/Adwaita-dark/scalable/apps
install -m 0644 %{fstab_srcdir}/resources/fstabulator_icon_dark.svg \
	%{buildroot}%{_datadir}/icons/Adwaita-dark/scalable/apps/fstabulator.svg
cat > %{buildroot}%{_datadir}/icons/Adwaita-dark/index.theme <<'EOF'
[Icon Theme]
Name=Adwaita-dark
Inherits=Adwaita,hicolor
Directories=scalable/apps

[scalable/apps]
Context=Applications
Size=128
MinSize=8
MaxSize=512
Type=Scalable
EOF

install -d %{buildroot}%{_datadir}/polkit-1/actions
cat > %{buildroot}%{_datadir}/polkit-1/actions/org.lapissea.FSTabulator.root-helper.policy <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE policyconfig PUBLIC
 "-//freedesktop//DTD PolicyKit Policy Configuration 1.0//EN"
 "http://www.freedesktop.org/standards/PolicyKit/1.0/policyconfig.dtd">
<policyconfig>
	<vendor>FSTabulator</vendor>
	<vendor_url>https://github.com/LapisSea/fstabulator</vendor_url>
	<icon_name>org.lapissea.FSTabulator</icon_name>
	<action id="org.lapissea.FSTabulator.root-helper">
		<description>FSTabulator is requesting system access</description>
		<message>FSTabulator needs administrator access to edit /etc/fstab, keep backups of it, and to mount, unmount or swap your drives.</message>
		<icon_name>org.lapissea.FSTabulator</icon_name>
		<defaults>
			<allow_any>auth_admin</allow_any>
			<allow_inactive>auth_admin</allow_inactive>
			<allow_active>auth_admin</allow_active>
		</defaults>
		<annotate key="org.freedesktop.policykit.exec.path">%{_bindir}/fstabulator</annotate>
		<annotate key="org.freedesktop.policykit.exec.argv1">--root-helper</annotate>
	</action>
</policyconfig>
EOF

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
